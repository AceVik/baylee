"""Opt-in `--train-candidate-column`: one input column of the candidate
projection learns and nothing else does.

Candidates read the board and never feed it, so the column reaches nothing but
the candidates that carry its slot. A candidate with the slot at zero multiplies
the column by zero: it gets no gradient and scores exactly as the initial
network does, which is what keeps every other decision untouched.
"""
import torch


def weight_of(net):
    return net.cand_proj[0].weight


def freeze_all_but_column(net, column, *, start_from_zero):
    """Leaves one column trainable; `start_from_zero` is false on a resume."""
    for parameter in net.parameters():
        parameter.requires_grad_(False)
    weight = weight_of(net)
    if start_from_zero:
        with torch.no_grad():
            weight[:, column] = 0
    weight.requires_grad_(True)
    keep = torch.zeros(weight.shape[1], dtype=weight.dtype)
    keep[column] = 1

    def only_the_column(grad):
        return grad * keep.to(grad.device)

    weight.register_hook(only_the_column)


def carries_column(host, column):
    """Whether a host batch has a candidate with the slot set: any other batch
    has no gradient to give, and Adam would only replay its momentum."""
    return bool((host["cand_feat"][..., column] != 0).any())
