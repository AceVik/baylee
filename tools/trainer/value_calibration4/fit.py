"""Optimize a detached copy of the head; the source network never participates."""
import copy
import numpy as np
import torch
from .features import predict
from .metrics import evaluate, training_priors, trajectory_weights


def fit(source_head, train, validation, device, epochs=20, lr=1e-4, seed=31431, batch_size=256):
    head = copy.deepcopy(source_head).to(device).float()
    for p in head.parameters():
        p.requires_grad_(True)
    optimizer = torch.optim.Adam(head.parameters(), lr=lr)
    rng = np.random.default_rng(seed)
    trajectories = train.records["trajectory"][train.indices]
    count = len(trajectories)
    weights = trajectory_weights(trajectories) * count
    priors, fallback = training_priors(train)
    initial = evaluate(head, validation, device, priors, fallback)
    best_score = initial["all"]["calibrated"]["trajectory_weighted"]["mse"]
    best_state = {k: v.detach().cpu().clone() for k, v in head.state_dict().items()}
    best_epoch = 0
    history = [dict(epoch=0, validation_trajectory_mse=best_score)]
    for epoch in range(1, epochs + 1):
        head.train()
        for positions in np.array_split(rng.permutation(count), max(1, (count+batch_size-1)//batch_size)):
            x, active, allied, target = train.tensor_batch(train.indices[positions], device)
            weight = torch.tensor(weights[positions], dtype=torch.float32, device=device)
            optimizer.zero_grad(set_to_none=True)
            loss = ((predict(head, x, active, allied) - target).square() * weight).mean()
            if not torch.isfinite(loss):
                raise FloatingPointError("Nonfinite critic calibration loss")
            loss.backward()
            torch.nn.utils.clip_grad_norm_(head.parameters(), 1.0, error_if_nonfinite=True)
            optimizer.step()
        metrics = evaluate(head, validation, device, priors, fallback)
        score = metrics["all"]["calibrated"]["trajectory_weighted"]["mse"]
        history.append(dict(epoch=epoch, validation_trajectory_mse=score))
        print(f"[value-fit] epoch={epoch} trajectory_val_mse={score:.6f} best={min(best_score,score):.6f}", flush=True)
        if score < best_score:
            best_score, best_epoch = score, epoch
            best_state = {k: v.detach().cpu().clone() for k, v in head.state_dict().items()}
    head.load_state_dict(best_state)
    return head, dict(selected_epoch=best_epoch, history=history,
                      objective="terminal side-utility MSE; equal trajectory weights, equal sampled frames within trajectory; no outcome balancing",
                      selection="validation trajectory-weighted MSE including unchanged source epoch0",
                      training_seatcount_priors={str(k): v for k, v in priors.items()},
                      training_prior_fallback=fallback,
                      train=evaluate(head, train, device, priors, fallback),
                      validation=evaluate(head, validation, device, priors, fallback))
