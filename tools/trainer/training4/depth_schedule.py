"""Versioned decision-depth contract shared by eager and planned execution."""
from dataclasses import asdict, dataclass
import hashlib
import json


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
        if self.deterministic_algorithms is not True:
            raise ValueError("Exact resume requires deterministic algorithms")
        if self.version != 1 or self.mode not in ("fixed", "legacy_frame_cycle"):
            raise ValueError("Unsupported depth schedule version/mode")
        if self.phase_rule != "optimizer_step_plus_frame_mod_period":
            raise ValueError("Unsupported depth schedule phase rule")
        if not self.positive_k or any(type(k) is not int or k < 1 or k > 8 for k in self.positive_k):
            raise ValueError("Decision depths must be positive K1..8; K0 is observation-only")
        if self.mode == "fixed" and len(self.positive_k) != 1:
            raise ValueError("Fixed depth needs exactly one K")
        if self.mode == "legacy_frame_cycle" and self.positive_k != (1, 2, 4, 8):
            raise ValueError("Legacy frame cycle is exactly K1/2/4/8")
        if self.validation_history != "complete_independent_reset_per_k":
            raise ValueError("Validation must reset independent complete histories for every K")
        if self.selection_k != 2 or self.report_k != (1, 2, 4, 8):
            raise ValueError("Validation contract requires selection K2 and reports K1/2/4/8")

    @classmethod
    def from_training(cls, thinking_steps=2, curriculum=False):
        return cls(mode="legacy_frame_cycle", positive_k=(1, 2, 4, 8)) if curriculum else cls(positive_k=(thinking_steps,))

    def depth(self, step, frame):
        return self.positive_k[(step + frame) % len(self.positive_k)]

    def to_dict(self):
        contract = asdict(self)
        contract["sha256"] = hashlib.sha256(json.dumps(contract, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
        return contract

    def check_resume(self, saved):
        if saved.get("depth_schedule") != self.to_dict():
            raise ValueError("Resume depth/validation schedule differs or is missing; use --init for a new weights-only continuation")

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
