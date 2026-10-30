"""Versioned decision-depth contract shared by eager and planned execution."""
from dataclasses import asdict, dataclass
import json
from trajectory_store4 import _native


@dataclass(frozen=True)
class DepthSchedule:
    version: int = 1
    mode: str = "fixed"
    positive_k: tuple[int, ...] = (2,)
    phase_rule: str = "optimizer_step_plus_frame_mod_period"
    selection_k: int = 2
    report_k: tuple[int, ...] = (1, 2, 4, 8)
    validation_history: str = "complete_independent_reset_per_k"
    deterministic_algorithms: bool = True

    def __post_init__(self):
        self.to_dict()

    @classmethod
    def from_training(cls, thinking_steps=2, curriculum=False):
        return cls(mode="legacy_frame_cycle", positive_k=(1, 2, 4, 8)) if curriculum else cls(positive_k=(thinking_steps,))

    def depth(self, step, frame):
        return self.positive_k[(step + frame) % len(self.positive_k)]

    def to_dict(self):
        contract=json.loads(_native(['training-contract'],json.dumps(
            dict(kind='depth',contract=asdict(self)),allow_nan=False).encode()))
        contract['positive_k']=tuple(contract['positive_k'])
        contract['report_k']=tuple(contract['report_k'])
        return contract

    def check_resume(self, saved):
        _native(['training-contract'],json.dumps(dict(kind='depth_resume',contract=asdict(self),
            saved={'depth_schedule':saved.get('depth_schedule')}),allow_nan=False).encode())

    def count_batch(self, batch, step, totals):
        """Count real valid decisions and their supervised policy weight on CPU."""
        for t in range(batch["step_valid"].shape[1]):
            k = self.depth(step, t)
            mask = batch["step_valid"][:, t].bool() & batch["is_decision"][:, t].bool()
            entry = totals.setdefault(str(k), {"decisions": 0, "policy_loss_weight": 0.0})
            entry["decisions"] += int(mask.sum())
            entry["policy_loss_weight"] += float(batch["loss_weight"][:, t][mask].double().sum())


def evaluate_depths(net, loader, device, schedule, evaluate):
    """Each evaluator call resets H and traverses the complete held-out history."""
    per_k = {str(k): evaluate(net, loader, device, k) for k in schedule.report_k}
    return {**per_k[str(schedule.selection_k)], "per_k": per_k,
            "selection_k": schedule.selection_k}
