"""Thin Torch model configuration bridge to native semantic encoder admission."""
import json
from trajectory_store4 import _native
DEFAULT_ENCODER = 7
PAYMENT_ENCODER = 8


def validate_encoder(version, *, model_version=5, relational_revision=2,
                     candidate_context_revision=1, global_feat_width=None):
    return json.loads(_native(['training-contract'],json.dumps(dict(kind='encoder',version=version,
        model_version=model_version,relational_revision=relational_revision,
        candidate_context_revision=candidate_context_revision,global_feat_width=global_feat_width),allow_nan=False).encode()))


def check_initial_encoder(source, target, explicit_from7=False, explicit_to9=False):
    return json.loads(_native(['training-contract'],json.dumps(dict(kind='initial_encoder',source=source,
        target=target,explicit_from7=explicit_from7,explicit_to9=explicit_to9),allow_nan=False).encode()))
