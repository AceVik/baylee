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
    categorical_weighting: str = 'symmetric_v1'

    def __post_init__(self):
        _native(['training-contract'],json.dumps(dict(kind='supervised_loss',contract=self.to_dict()),
            allow_nan=False).encode())

    def validate_execution(self, *, reference_execution, init_state):
        _native(['training-contract'], json.dumps(dict(kind='supervised_execution',
            contract=self.to_dict(), reference_execution=reference_execution,
            init_state=bool(init_state)), allow_nan=False).encode())

    def to_dict(self):
        contract = dict(version=self.version, outcome_weighting=self.outcome_weighting,
                    normalization=self.normalization, policy_head_weights=list(self.policy_head_weights),
                    detach_value_input=self.detach_value_input, detach_aux_input=self.detach_aux_input,
                    value_weight=self.value_weight, auxiliary_weight=self.auxiliary_weight)
        # Keep the canonical loss4 contract byte-for-byte unchanged.
        if self.version == 5 or self.categorical_weighting != 'symmetric_v1':
            contract['categorical_weighting'] = self.categorical_weighting
        return contract

    @classmethod
    def from_dict(cls, value):
        value=json.loads(_native(['training-contract'],json.dumps(
            dict(kind='supervised_saved_loss',contract=value),allow_nan=False).encode()))
        if 'policy_head_weights' in value:
            value['policy_head_weights']=tuple(value['policy_head_weights'])
        return cls(**value)
