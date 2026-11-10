"""Tensor update boundary shared by ordinary and frozen-trunk training."""
import torch

from .optimizer_groups import apply_relational_warmup


def update_parameters(loss, net, optimizer, checkpoint_writer, schedule, update_number, profile=None):
    """Consume a loss only when it reaches trainable parameters.

    Passive recurrent observations still advance memory in the caller. With a
    frozen trunk they have no trainable loss; Adam, weight decay, and the update
    schedule must all stay unchanged. The caller clears gradients each batch.
    `profile` (opt-in) only stamps phase boundaries.
    """
    if not torch.isfinite(loss):
        raise FloatingPointError("Non-finite training loss")
    if profile is not None:
        profile.mark("finite_check")
    if not loss.requires_grad:
        return False
    loss.backward()
    if profile is not None:
        profile.mark("backward")
    torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0, error_if_nonfinite=True)
    if profile is not None:
        profile.mark("clip")
    checkpoint_writer.before_mutation()
    apply_relational_warmup(optimizer, schedule, update_number)
    optimizer.step()
    if profile is not None:
        profile.mark("optimizer")
    return True
