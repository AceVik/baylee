"""Execution-only offload of nonreentrant checkpoint inputs, not layer weights."""
import torch
from torch.utils.checkpoint import checkpoint


def checkpoint_activation(function, *args, offload_inputs=False):
    # Checkpoint's internal hooks own saves inside the function. This outer
    # context therefore offloads its input saver only, avoiding whole-forward
    # copies of persistent parameters. CPU/default paths are exactly unchanged.
    if offload_inputs and any(isinstance(arg, torch.Tensor) and arg.is_cuda for arg in args):
        with torch.autograd.graph.save_on_cpu(pin_memory=True):
            return checkpoint(function, *args, use_reentrant=False)
    return checkpoint(function, *args, use_reentrant=False)
