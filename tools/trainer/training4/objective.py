"""Versioned learning/selection semantics, separate from model architecture."""
import torch
import torch.nn.functional as F

LOSS_VERSION = 2


def selection_score(metrics, selection):
    if selection == "policy":
        return metrics["val_pol_loss"]
    if selection == "combined":
        return metrics["val_pol_loss"] + 2.0 * metrics["val_mse"]
    raise ValueError(f"Unknown checkpoint selection: {selection}")


def check_resume_objective(saved, selection):
    if saved.get("training_loss_version", 1) != LOSS_VERSION:
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
