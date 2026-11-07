"""Opt-in `--train-rules-only` and `--anchor-l2`.

`--train-rules-only` lets the rule encoder learn through the network as
`--init` gave it: the losses of play reach every card's rule embedding, and
nothing else moves. Cards change meaning for the trunk only where their
embedding does.

`--anchor-l2` adds the squared distance of every trainable weight from its
`--init` value to the loss (L2-SP), so a fine-tune can move only as far as the
data pays for. It replaces weight decay, which would pull toward zero instead.
"""
import torch


def freeze_all_but_rules(net):
    """Leaves the rule encoder trainable and returns its parameter names."""
    names = []
    for name, parameter in net.named_parameters():
        trainable = name.startswith("rules.")
        parameter.requires_grad_(trainable)
        if trainable:
            names.append(name)
    if not names:
        raise ValueError("The network has no rule encoder to train")
    return names


class ParameterAnchor:
    """The trainable weights as they started, and the distance from them."""

    def __init__(self, net, weight):
        if not weight > 0:
            raise ValueError("An anchor needs a positive weight")
        self.weight = weight
        self.pairs = [(p, p.detach().clone()) for p in net.parameters() if p.requires_grad]

    def penalty(self):
        return self.weight * sum((p - start).square().sum() for p, start in self.pairs)

    def distance(self):
        with torch.no_grad():
            return float(sum((p - start).square().sum() for p, start in self.pairs).sqrt())
