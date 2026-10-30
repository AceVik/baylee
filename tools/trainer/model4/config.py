"""V4 tensor contracts and configurable model dimensions."""
from dataclasses import asdict, dataclass
import torch

RECURRENT_SLOTS = 9
MAX_SEATS = 8
MAX_ENTITIES = 1024
MAX_EDGES = 8192
MAX_CANDIDATES = 65535
MAX_PAIRS = 65535
ENCODER_VERSION = 7
DSL_VERSION = 2
UNKNOWN_CARD = 2**32 - 1
TRAJECTORY_VERSION = 2
SAMPLING_VERSION = 2
MODEL_VERSION = 3
ENTITY_FEAT_WIDTH = 128
CARD_DSL_WIDTH = 160
SEAT_FEAT_WIDTH = 64
GLOBAL_FEAT_WIDTH = 80
CAND_FEAT_WIDTH = 48
NUM_EDGE_TYPES = 14

# Shared seat_feat column indices (must match `crates/baylee-train/src/features4.rs:encode_seats4`)
SEAT_FEAT_PRESENT = 0
SEAT_FEAT_SELF = 1
SEAT_FEAT_TEAMMATE = 2
SEAT_FEAT_OPPONENT = 3
SEAT_FEAT_LOST = 4

INPUT_NAMES = (
    "entity_feat",
    "entity_dsl",
    "entity_mask",
    "seat_feat",
    "global_feat",
    "edge_index",
    "edge_type",
    "edge_mask",
    "cand_feat",
    "cand_entity_idx",
    "cand_seat_idx",
    "cand_mask",
    "pair_index",
    "pair_mask",
    "cand_dsl",
    "entity_rules",
    "cand_rules",
    "h_in",
    "thinking_steps",
)

OUTPUT_NAMES = (
    "h_out",
    "cand_logits",
    "pair_logits",
    "scalar_pred",
    "value",
)


@dataclass
class Config4:
    global_feat_width: int = GLOBAL_FEAT_WIDTH
    structure_width: int = 128
    d_model: int = 512
    n_layers: int = 8
    n_heads: int = 8
    d_ff: int = 2048
    dropout: float = 0.0
    activation_checkpointing: bool = False
    default_thinking_steps: int = 2
    max_thinking_steps: int = 8

    def __post_init__(self):
        if type(self.global_feat_width) is not int or self.global_feat_width not in (80, 82):
            raise ValueError('Unsupported global feature width')

    def to_dict(self) -> dict:
        return asdict(self)


def symlog(x: torch.Tensor) -> torch.Tensor:
    return torch.sign(x) * torch.log1p(torch.abs(x))
