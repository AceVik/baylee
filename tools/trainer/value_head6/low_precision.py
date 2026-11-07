"""AdamW with no 32-bit copy of anything it keeps between steps.

Weights live in bfloat16, and so may the moments. A step's arithmetic runs in
float32 temporaries, as a fused kernel would hold it in registers; nothing of it
survives the step. Plain rounding loses every update below half a bfloat16 step
of the weight, so `stochastic` rounds up or down with the odds that keep the
expected value, and small updates arrive on average.
"""
from dataclasses import dataclass

import torch


@dataclass(frozen=True)
class Numerics:
    """How the head's weights and optimizer state are stored between steps."""
    weights: str = "float32"
    rounding: str = "nearest"
    state: str = "float32"

    @property
    def dtype(self):
        return torch.bfloat16 if self.weights == "bfloat16" else torch.float32


def stochastic_bfloat16(values):
    """Float32 to bfloat16, up or down at random, the expected value kept.

    The low sixteen bits are what bfloat16 drops. Adding sixteen random bits
    before dropping them carries into the kept bits with exactly the odds of
    the part dropped. Raw bits hold the magnitude, so both signs round alike.
    """
    bits = values.contiguous().view(torch.int32)
    noise = torch.randint(0, 1 << 16, bits.shape, dtype=torch.int32, device=bits.device)
    return ((bits + noise) & -65536).view(torch.float32).to(torch.bfloat16)


class LowPrecisionAdamW(torch.optim.Optimizer):
    def __init__(self, groups, lr, numerics, betas=(.9, .999), eps=1e-8):
        super().__init__(groups, dict(lr=lr, betas=betas, eps=eps, weight_decay=0.0))
        self.numerics = numerics

    def _store(self, target, value):
        rounded = (stochastic_bfloat16(value) if self.numerics.rounding == "stochastic" and target.dtype == torch.bfloat16
                   else value.to(target.dtype))
        target.copy_(rounded)

    @torch.no_grad()
    def step(self, closure=None):
        state_dtype = torch.bfloat16 if self.numerics.state == "bfloat16" else torch.float32
        for group in self.param_groups:
            beta1, beta2 = group["betas"]
            for parameter in group["params"]:
                if parameter.grad is None:
                    continue
                state = self.state[parameter]
                if not state:
                    state["step"] = 0
                    state["first"] = torch.zeros_like(parameter, dtype=state_dtype)
                    state["second"] = torch.zeros_like(parameter, dtype=state_dtype)
                state["step"] += 1
                gradient = parameter.grad.float()
                first = state["first"].float().mul_(beta1).add_(gradient, alpha=1 - beta1)
                second = state["second"].float().mul_(beta2).addcmul_(gradient, gradient, value=1 - beta2)
                self._store(state["first"], first)
                self._store(state["second"], second)
                scale = (1 - beta2 ** state["step"]) ** .5 / (1 - beta1 ** state["step"])
                update = first / (second.sqrt() + group["eps"]) * (group["lr"] * scale)
                weight = parameter.float()
                weight.mul_(1 - group["lr"] * group["weight_decay"]).sub_(update)
                self._store(parameter, weight)
