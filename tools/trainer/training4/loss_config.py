"""Explicit opt-in supervised objective contracts; legacy version4 remains default."""
from dataclasses import dataclass
import json
from trajectory_store4 import _native


@dataclass(frozen=True)
class SupervisedLossConfig:
    version: int = 4
    outcome_weighting: bool = True
    normalization: str = 'legacy_timestep'
    policy_head_weights: tuple = (1., 1., 1., 1.)
    detach_value_input: bool = False
    detach_aux_input: bool = False
    value_weight: float = .5
    auxiliary_weight: float = .25

    def __post_init__(self):
        _native(['training-contract'],json.dumps(dict(kind='supervised_loss',contract=self.to_dict()),
            allow_nan=False).encode())

    def to_dict(self):
        return dict(version=self.version, outcome_weighting=self.outcome_weighting,
                    normalization=self.normalization, policy_head_weights=list(self.policy_head_weights),
                    detach_value_input=self.detach_value_input, detach_aux_input=self.detach_aux_input,
                    value_weight=self.value_weight, auxiliary_weight=self.auxiliary_weight)

    @classmethod
    def from_dict(cls, value):
        value=dict(value)
        if 'policy_head_weights' in value:
            value['policy_head_weights']=tuple(value['policy_head_weights'])
        return cls(**value)
