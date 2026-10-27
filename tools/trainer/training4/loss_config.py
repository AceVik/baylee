"""Explicit opt-in supervised objective contracts; legacy version4 remains default."""
from dataclasses import dataclass


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
        if self.version not in (4, 5) or type(self.outcome_weighting) is not bool:
            raise ValueError('Invalid supervised loss version/outcome flag')
        if type(self.detach_value_input) is not bool or type(self.detach_aux_input) is not bool:
            raise ValueError('Detachment flags must be boolean')
        if self.normalization not in ('legacy_timestep', 'unroll_per_head'):
            raise ValueError('Unsupported supervised normalization')
        if tuple(self.policy_head_weights) != (1., 1., 1., 1.) or self.value_weight != .5 or self.auxiliary_weight != .25:
            raise ValueError('This ablation fixes policy/value/aux weights to1/.5/.25')
        if self.version == 4 and (not self.outcome_weighting or self.normalization != 'legacy_timestep' or self.detach_value_input or self.detach_aux_input):
            raise ValueError('Loss4 cannot enable new ablations')

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
