"""Versioned learning/selection semantics, separate from model architecture."""
import torch
import torch.nn.functional as F

# v4: symmetric categorical CE and ordinary Bernoulli BCE, unit head weights.
# Existing explicit per-step/outcome weights remain; this is weighted imitation,
# not a guarantee of calibrated deployment probabilities.
LOSS_VERSION = 4


def selection_score(metrics, selection):
    if selection == "policy":
        return metrics["val_pol_loss"]
    if selection == "combined":
        return metrics["val_pol_loss"] + 2.0 * metrics["val_mse"]
    raise ValueError(f"Unknown checkpoint selection: {selection}")


def check_resume_objective(saved, selection, loss_config=None):
    from .loss_config import SupervisedLossConfig
    expected = loss_config or SupervisedLossConfig()
    recorded = saved.get('supervised_loss_config')
    if recorded is not None and expected.version == 5 and set(recorded) != set(expected.to_dict()):
        raise ValueError('Incomplete supervised loss configuration')
    if recorded is not None and SupervisedLossConfig.from_dict(recorded) != expected:
        raise ValueError('Resume supervised loss configuration differs')
    if recorded is None and expected.version != 4:
        raise ValueError('Explicit supervised loss configuration required for version5')
    if saved.get("training_loss_version", 1) != expected.version:
        raise ValueError("Resume loss semantics differ; use --init for a new learning experiment")
    if saved.get("checkpoint_selection", "combined") != selection:
        raise ValueError("Resume checkpoint selection differs from the saved scoring protocol")


def choice_statistics(logits, candidates, single, target, weight):
    """Unweighted NLL and accuracy on decisions with a real alternative.

    Passing when alternatives exist counts; a forced non-pass does not.
    All history still updates memory and receives value/auxiliary supervision.
    """
    eligible = single & (candidates.sum(-1) > 1) & (weight > 0)
    safe_target = torch.where(single, target, 0)
    nll = F.cross_entropy(logits.float(), safe_target, reduction="none")
    return torch.stack(((nll * eligible).sum(),
                        ((logits.argmax(-1) == safe_target) & eligible).sum(), eligible.sum()))
